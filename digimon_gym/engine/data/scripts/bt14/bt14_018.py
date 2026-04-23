from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_018(CardScript):
    """BT14-018 Goldramon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] Play 1 [Amon of Crimson Flame] (Digimon/Red/6000 DP/<Rush>) Token and 1 [Umon of Blue Thunder] (Digimon/Yellow/6000 DP/<Blocker>) Token.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("BT14-018 Play tokens")
        effect0.set_effect_description("[On Play] Play 1 [Amon of Crimson Flame] (Digimon/Red/6000 DP/<Rush>) Token and 1 [Umon of Blue Thunder] (Digimon/Yellow/6000 DP/<Blocker>) Token.")
        effect0.is_on_play = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Play Token"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                game.effect_play_token(player, 'amon')
                game.effect_play_token(player, 'umon')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Play 1 [Amon of Crimson Flame] (Digimon/Red/6000 DP/<Rush>) Token and 1 [Umon of Blue Thunder] (Digimon/Yellow/6000 DP/<Blocker>) Token.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT14-018 Play tokens")
        effect1.set_effect_description("[When Digivolving] Play 1 [Amon of Crimson Flame] (Digimon/Red/6000 DP/<Rush>) Token and 1 [Umon of Blue Thunder] (Digimon/Yellow/6000 DP/<Blocker>) Token.")
        effect1.is_when_digivolving = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Token"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                game.effect_play_token(player, 'amon')
                game.effect_play_token(player, 'umon')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.BeforePayCost
        # [All Turns] When this Digimon would digivolve or leave the battle area, delete all of your [Amon of Crimson Flame] and [Umon of Blue Thunder]. If this effect deletes, <Recovery +1 (Deck)>.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.BeforePayCost)
        effect2.set_effect_name("BT14-018 Delete tokens and Recovery +1 (Deck)")
        effect2.set_effect_description("[All Turns] When this Digimon would digivolve or leave the battle area, delete all of your [Amon of Crimson Flame] and [Umon of Blue Thunder]. If this effect deletes, <Recovery +1 (Deck)>.")
        effect2.set_hash_string("Recovery1_BT14_018")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete own Amon/Umon tokens; if any deleted, Recovery +1"""
            player = ctx.get('player')
            if not player:
                return

            deleted_any = False
            for p in list(getattr(player, 'battle_area', [])):
                if not getattr(p, 'is_digimon', False):
                    continue
                name = getattr(p, 'get_name', lambda: '')()
                if name in ("Amon of Crimson Flame", "Umon of Blue Thunder"):
                    player.delete_permanent(p)
                    deleted_any = True

            if deleted_any:
                player.recovery(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.WhenRemoveField
        # [All Turns] When this Digimon would digivolve or leave the battle area, delete all of your [Amon of Crimson Flame] and [Umon of Blue Thunder]. If this effect deletes, <Recovery +1 (Deck)>.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenRemoveField)
        effect3.set_effect_name("BT14-018 Delete tokens and Recovery +1 (Deck)")
        effect3.set_effect_description("[All Turns] When this Digimon would digivolve or leave the battle area, delete all of your [Amon of Crimson Flame] and [Umon of Blue Thunder]. If this effect deletes, <Recovery +1 (Deck)>.")
        effect3.set_hash_string("Recovery1_BT14_018")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Delete own Amon/Umon tokens; if any deleted, Recovery +1"""
            player = ctx.get('player')
            if not player:
                return

            deleted_any = False
            for p in list(getattr(player, 'battle_area', [])):
                if not getattr(p, 'is_digimon', False):
                    continue
                name = getattr(p, 'get_name', lambda: '')()
                if name in ("Amon of Crimson Flame", "Umon of Blue Thunder"):
                    player.delete_permanent(p)
                    deleted_any = True

            if deleted_any:
                player.recovery(1)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
