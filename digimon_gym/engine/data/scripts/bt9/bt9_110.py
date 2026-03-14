from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT9_110(CardScript):
    """BT9-110 X Program | Option (Black, Cost 8)

    While you have a Digimon with [Dex] or [DeathX] in its name, you can
        ignore this card's color requirements.
    [Main] Delete 1 Digimon without [X Antibody] trait. If there are 3 or
        more Digimon in play, delete all Digimon without [X Antibody] trait
        instead.
    [Security] Delete 1 of your opponent's Digimon without [X Antibody] trait.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _has_dex_or_deathx(player) -> bool:
            """Check if player has a Digimon with [Dex] or [DeathX] in name."""
            if not player:
                return False
            for p in player.battle_area:
                if not p.is_digimon:
                    continue
                names = getattr(p.top_card, 'card_names', []) or [] if p.top_card else []
                for n in names:
                    if 'Dex' in n or 'DeathX' in n:
                        return True
            return False

        def _lacks_x_antibody(perm) -> bool:
            """Check if a permanent lacks [X Antibody] trait."""
            if not perm.is_digimon:
                return False
            tc = perm.top_card
            if not tc:
                return True
            traits = getattr(tc, 'card_traits', []) or []
            return not any('X Antibody' in t for t in traits)

        # --- Effect 0: Color requirement bypass ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT9-110 Ignore color with Dex/DeathX")
        effect0.set_effect_description(
            "While you have a Digimon with [Dex] or [DeathX] in its name, "
            "you can ignore this card's color requirements."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            owner = getattr(card, 'owner', None)
            return _has_dex_or_deathx(owner)
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            pass
        effect0.set_on_process_callback(process0)
        effect0._match_color_requirement = False
        effects.append(effect0)

        # --- Effect 1: [Main] Delete Digimon without [X Antibody] ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("BT9-110 Delete non-X Antibody Digimon")
        effect1.set_effect_description(
            "[Main] Delete 1 Digimon without [X Antibody] trait. If there are "
            "3 or more Digimon in play, delete all Digimon without [X Antibody] "
            "trait instead."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy

            # Count total Digimon in play (both sides)
            total_digimon = 0
            total_digimon += sum(1 for p in player.battle_area if p.is_digimon)
            if enemy:
                total_digimon += sum(1 for p in enemy.battle_area if p.is_digimon)

            if total_digimon >= 3:
                # Delete ALL Digimon without [X Antibody] trait (both sides)
                for p in list(player.battle_area):
                    if _lacks_x_antibody(p):
                        player.delete_permanent(p)
                if enemy:
                    for p in list(enemy.battle_area):
                        if _lacks_x_antibody(p):
                            enemy.delete_permanent(p)
            else:
                # Delete 1 Digimon without [X Antibody] (player chooses from either side)
                # Simplified: select from opponent first
                if enemy:
                    opp_targets = [p for p in enemy.battle_area if _lacks_x_antibody(p)]
                    own_targets = [p for p in player.battle_area if _lacks_x_antibody(p)]
                    if opp_targets:
                        def on_delete(target_perm):
                            enemy.delete_permanent(target_perm)
                        game.effect_select_opponent_permanent(
                            player, on_delete,
                            filter_fn=_lacks_x_antibody,
                            is_optional=False,
                            prompt="Delete 1 Digimon without [X Antibody] trait.")
                    elif own_targets:
                        def on_delete_own(target_perm):
                            player.delete_permanent(target_perm)
                        game.effect_select_own_permanent(
                            player, on_delete_own,
                            filter_fn=_lacks_x_antibody,
                            is_optional=False,
                            prompt="Delete 1 of your Digimon without [X Antibody] trait.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [Security] Delete 1 opponent Digimon without [X Antibody] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT9-110 Security: Delete opponent non-X Antibody")
        effect2.set_effect_description(
            "[Security] Delete 1 of your opponent's Digimon without [X Antibody] trait."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            opp_targets = [p for p in enemy.battle_area if _lacks_x_antibody(p)]
            if opp_targets:
                def on_delete(target_perm):
                    enemy.delete_permanent(target_perm)
                game.effect_select_opponent_permanent(
                    player, on_delete,
                    filter_fn=_lacks_x_antibody,
                    is_optional=False,
                    prompt="Delete 1 of your opponent's Digimon without [X Antibody] trait.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
