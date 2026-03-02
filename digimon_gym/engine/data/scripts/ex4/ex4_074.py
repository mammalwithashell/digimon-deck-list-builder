from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX4_074(CardScript):
    """EX4-074 ShineGreymon: Ruin Mode | Lv.7 Purple/Yellow Digimon

    [When Digivolving] [On Deletion] Until the end of your opponent's next
        turn, all of your opponent's Digimon get -5000DP.
    [End of Attack] Delete this Digimon and 1 of your opponent's Digimon,
        and Recovery +1 (Deck). Then, if you have a Tamer in play, hatch 1
        Digi-Egg card to an empty space in your breeding area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [When Digivolving] Opponent Digimon get -5000 DP ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX4-074 When Digivolving: opponent -5000 DP")
        effect0.set_effect_description(
            "[When Digivolving] Until the end of your opponent's next turn, "
            "all of your opponent's Digimon get -5000DP."
        )
        effect0.is_when_digivolving = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Apply -5000 DP to all opponent Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            for perm in enemy.battle_area:
                if perm.is_digimon:
                    perm.dp_modifier -= 5000
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [On Deletion] Opponent Digimon get -5000 DP ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDestroyedAnyone)
        effect1.set_effect_name("EX4-074 On Deletion: opponent -5000 DP")
        effect1.set_effect_description(
            "[On Deletion] Until the end of your opponent's next turn, "
            "all of your opponent's Digimon get -5000DP."
        )
        effect1.is_on_deletion = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Apply -5000 DP to all opponent Digimon on deletion."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            for perm in enemy.battle_area:
                if perm.is_digimon:
                    perm.dp_modifier -= 5000
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [End of Attack] Delete self + 1 opponent + recovery + hatch ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEndAttack)
        effect2.set_effect_name("EX4-074 End of Attack: delete self+opponent, recovery, hatch")
        effect2.set_effect_description(
            "[End of Attack] Delete this Digimon and 1 of your opponent's "
            "Digimon, and Recovery +1 (Deck). Then, if you have a Tamer in "
            "play, hatch 1 Digi-Egg card."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card() if card else None
            ctx_perm = context.get('attacking_permanent') or context.get('permanent')
            if perm and ctx_perm and perm != ctx_perm:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Delete self and 1 opponent Digimon, recovery +1, hatch if tamer."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return

            # Delete this Digimon
            perm = card.permanent_of_this_card() if card else None
            if perm and perm in player.battle_area:
                player.delete_permanent(perm)

            # Delete 1 opponent Digimon
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if opp_digimon:
                target = opp_digimon[0]
                enemy.delete_permanent(target)

            # Recovery +1
            player.recovery(1)

            # If you have a tamer, hatch 1 Digi-Egg
            has_tamer = any(
                p.is_tamer for p in player.battle_area
            )
            if has_tamer and player.egg_deck:
                # Hatch: move top egg to breeding area if empty
                if not player.breeding_area:
                    from ....core.permanent import Permanent
                    egg_card = player.egg_deck.pop(0)
                    egg_perm = Permanent([egg_card])
                    if game:
                        egg_perm.turn_played = game.turn_count
                        egg_perm._owner_game = game
                    player.breeding_area.append(egg_perm)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
