from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT1_089(CardScript):
    """BT1-089 Mimi Tachikawa | Tamer"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Effect 0: [Start of Your Turn] If you have 2 or less memory, set it to 3.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT1-089 Set memory to 3")
        effect0.set_effect_description(
            "[Start of Your Turn] If you have 2 or less memory, set it to 3."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and game.memory <= 2:
                game.memory = 3

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Effect 1: [Main] If you have a level 5 or higher green Digimon in play,
        # by suspending this Tamer, hatch 1 Digi-Egg card to an empty space in
        # your breeding area, or move 1 level 3 or higher Digimon from your
        # breeding area to your battle area.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDeclaration)
        effect1._is_field_main = True
        effect1.set_effect_name("BT1-089 Main: Hatch or move from breeding")
        effect1.set_effect_description(
            "[Main] If you have a level 5 or higher green Digimon in play, "
            "by suspending this Tamer, hatch 1 Digi-Egg card or move 1 level 3+ "
            "Digimon from breeding area to battle area."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm is None or perm.is_suspended:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            owner = card.owner
            from ....data.enums import CardColor
            has_lv5_green = any(
                p.is_digimon and (p.level or 0) >= 5
                and CardColor.Green in (getattr(p.top_card, 'card_colors', []) or [])
                for p in owner.battle_area
            )
            if not has_lv5_green:
                return False
            # Must be able to hatch or move
            can_hatch = (owner.breeding_area is None and len(owner.digitama_library_cards) > 0)
            can_move = (owner.breeding_area is not None and (owner.breeding_area.level or 0) >= 3)
            return can_hatch or can_move

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return
            # Suspend this Tamer
            perm.suspend()

            can_hatch = (player.breeding_area is None and len(player.digitama_library_cards) > 0)
            can_move = (player.breeding_area is not None and (player.breeding_area.level or 0) >= 3)

            if can_hatch and can_move:
                # Let player choose: 0 = hatch, 1 = move
                def on_choice(choice: int):
                    if choice == 0:
                        player.hatch()
                    else:
                        player.move_from_breeding()
                game.effect_choose_branch(
                    player, 2, on_choice,
                    prompt="Choose: (0) Hatch a Digi-Egg, or (1) Move from breeding to battle area."
                )
            elif can_hatch:
                player.hatch()
            elif can_move:
                player.move_from_breeding()

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Effect 2: [Security] Play this card without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT1-089 Security: Play free")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
