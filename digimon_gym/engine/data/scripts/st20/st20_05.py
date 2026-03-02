from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST20_05(CardScript):
    """ST20-05 Gatomon | Lv.4 Yellow Digimon

    [Security] At the end of the battle, play this card without paying the cost.
    [On Play] [When Digivolving] Give 2 of your opponent's Digimon
        <Security A. -1> until their turn ends.
    Inherited Effect [When Attacking] [Once Per Turn] 1 of your opponent's
        Digimon gets -2000 DP for the turn.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Security] Play this card without paying the cost ---
        effect0 = ICardEffect()
        effect0.set_effect_name("ST20-05 Security: Play free")
        effect0.set_effect_description(
            "[Security] At the end of the battle, play this card without "
            "paying the cost."
        )
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Play this card from security without paying cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and card:
                player.play_card_from_source(card, pay_cost=False)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [On Play] Give 2 opponent Digimon Security A. -1 ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("ST20-05 On Play: opponent Security A. -1")
        effect1.set_effect_description(
            "[On Play] Give 2 of your opponent's Digimon <Security A. -1> "
            "until their turn ends."
        )
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Give 2 opponent Digimon Security Attack -1."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            targets = [p for p in enemy.battle_area if p.is_digimon]
            for perm in targets[:2]:
                perm.security_attack_modifier = getattr(perm, 'security_attack_modifier', 0) - 1
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [When Digivolving] Give 2 opponent Digimon Security A. -1 ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("ST20-05 When Digivolving: opponent Security A. -1")
        effect2.set_effect_description(
            "[When Digivolving] Give 2 of your opponent's Digimon "
            "<Security A. -1> until their turn ends."
        )
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Give 2 opponent Digimon Security Attack -1."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            targets = [p for p in enemy.battle_area if p.is_digimon]
            for perm in targets[:2]:
                perm.security_attack_modifier = getattr(perm, 'security_attack_modifier', 0) - 1
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: Inherited [When Attacking] [Once Per Turn] -2000 DP ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnTappedAnyone)
        effect3.set_effect_name("ST20-05 Inherited: opponent -2000 DP")
        effect3.set_effect_description(
            "[When Attacking] [Once Per Turn] 1 of your opponent's Digimon "
            "gets -2000 DP for the turn."
        )
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("WhenAttacking_ST20-05_MinusDP")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card() if card else None
            ctx_perm = context.get('attacking_permanent') or context.get('permanent')
            if perm and ctx_perm and perm != ctx_perm:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Give 1 opponent Digimon -2000 DP."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy
            if not enemy:
                return
            targets = [p for p in enemy.battle_area if p.is_digimon]
            if targets:
                targets[0].dp_modifier -= 2000
        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
