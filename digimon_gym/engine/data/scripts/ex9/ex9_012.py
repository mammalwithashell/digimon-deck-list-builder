from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_012(CardScript):
    """EX9-012 MetalGreymon: Alterous Mode | Lv.5 Red/Purple

    Alt-digi 1: [Digivolve] [MetalGreymon]: Cost 1 (exact name, any level)
    Alt-digi 2: [Digivolve] Lv.4 w/[Greymon] in name or w/[ADVENTURE] trait: Cost 3
    [On Play] [When Digivolving] Delete 1 of your opponent's Digimon with 8000 DP or less.
    [Your Turn] When any of your Digimon or Tamers with [Garurumon] or [Tai Kamiya]
        in their names are played or any of your Digimon digivolve into a Digimon
        with [Garurumon] in its name, this Digimon may digivolve into a Digimon
        card with [Greymon] in its name in the hand without paying the cost.
    Inherited: [Your Turn] This Digimon gets +4000 DP.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Alt-digi from [MetalGreymon] (exact name, any level) for cost 1 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX9-012 Alt digivolve from MetalGreymon")
        effect0.set_effect_description("[Digivolve] [MetalGreymon]: Cost 1")
        effect0._alt_digi_cost = 1
        effect0._alt_digi_name = "MetalGreymon"
        effect0._alt_digi_exact = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Alt-digi from Lv.4 w/[Greymon] in name for cost 3 ---
        effect1 = ICardEffect()
        effect1.set_effect_name("EX9-012 Alt digivolve from Lv.4 Greymon")
        effect1.set_effect_description("[Digivolve] Lv.4 w/[Greymon] in name: Cost 3")
        effect1._alt_digi_cost = 3
        effect1._alt_digi_level = 4
        effect1._alt_digi_name = "Greymon"

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: Alt-digi from Lv.4 w/[ADVENTURE] trait for cost 3 ---
        effect2 = ICardEffect()
        effect2.set_effect_name("EX9-012 Alt digivolve from Lv.4 ADVENTURE")
        effect2.set_effect_description("[Digivolve] Lv.4 w/[ADVENTURE] trait: Cost 3")
        effect2._alt_digi_cost = 3
        effect2._alt_digi_level = 4
        effect2._alt_digi_trait = "ADVENTURE"

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # --- Shared delete process for On Play / When Digivolving ---
        def _delete_opponent_digimon(ctx: Dict[str, Any]):
            """Delete 1 opponent Digimon with DP <= 8000."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def delete_filter(p):
                return p.is_digimon and p.dp is not None and p.dp <= 8000

            def on_delete(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.delete_permanent(target_perm)

            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=delete_filter, is_optional=False,
                prompt="Delete 1 of your opponent's Digimon with 8000 DP or less."
            )

        # --- Effect 3: [On Play] Delete 1 opponent Digimon with 8000 DP or less ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX9-012 On Play: Delete opponent Digimon DP<=8000")
        effect3.set_effect_description(
            "[On Play] Delete 1 of your opponent's Digimon with 8000 DP or less."
        )
        effect3.is_on_play = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_delete_opponent_digimon)
        effects.append(effect3)

        # --- Effect 4: [When Digivolving] Delete 1 opponent Digimon with 8000 DP or less ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("EX9-012 When Digivolving: Delete opponent Digimon DP<=8000")
        effect4.set_effect_description(
            "[When Digivolving] Delete 1 of your opponent's Digimon with 8000 DP or less."
        )
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_delete_opponent_digimon)
        effects.append(effect4)

        # --- Effect 5: [Your Turn] Reactive digivolve — play observer ---
        # Triggers when your Digimon or Tamer with [Garurumon] or [Tai Kamiya] is played.
        effect5 = ICardEffect()
        effect5.set_effect_name("EX9-012 Reactive digivolve on Garurumon/Tai Kamiya play")
        effect5.set_effect_description(
            "[Your Turn] When any of your Digimon or Tamers with [Garurumon] or "
            "[Tai Kamiya] in their names are played, this Digimon may digivolve "
            "into a Digimon card with [Greymon] in its name in the hand without "
            "paying the cost."
        )
        effect5.is_optional = True
        effect5._is_play_observer = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # The played permanent must have [Garurumon] or [Tai Kamiya] in name
            played_perm = context.get('played_permanent')
            if not played_perm or not played_perm.top_card:
                return False
            # Must be our permanent
            if played_perm.owner != card.owner:
                return False
            has_garurumon = played_perm.contains_card_name('Garurumon')
            has_tai_kamiya = played_perm.contains_card_name('Tai Kamiya')
            if not (has_garurumon or has_tai_kamiya):
                return False
            return True
        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Digivolve this Digimon into a [Greymon] card from hand for free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            def hand_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return c.contains_card_name('Greymon')

            game.effect_digivolve_from_hand(
                player, perm, hand_filter, cost_override=0, is_optional=True,
                prompt="Select a Digimon card with [Greymon] in its name to digivolve into."
            )

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # --- Effect 6: [Your Turn] Reactive digivolve — digivolve observer ---
        # Triggers when your Digimon digivolves into a Digimon with [Garurumon] in name.
        effect6 = ICardEffect()
        effect6.set_effect_name("EX9-012 Reactive digivolve on Garurumon digivolve")
        effect6.set_effect_description(
            "[Your Turn] When any of your Digimon digivolve into a Digimon with "
            "[Garurumon] in its name, this Digimon may digivolve into a Digimon "
            "card with [Greymon] in its name in the hand without paying the cost."
        )
        effect6.is_optional = True
        effect6._is_digivolve_observer = True

        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # The digivolved permanent must have [Garurumon] in name
            digi_perm = context.get('digivolved_permanent')
            if not digi_perm or not digi_perm.top_card:
                return False
            # Must be our permanent
            if digi_perm.owner != card.owner:
                return False
            if not digi_perm.contains_card_name('Garurumon'):
                return False
            return True
        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Digivolve this Digimon into a [Greymon] card from hand for free."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return

            def hand_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return c.contains_card_name('Greymon')

            game.effect_digivolve_from_hand(
                player, perm, hand_filter, cost_override=0, is_optional=True,
                prompt="Select a Digimon card with [Greymon] in its name to digivolve into."
            )

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        # --- Effect 7: Inherited [Your Turn] +4000 DP ---
        effect7 = ICardEffect()
        effect7.set_effect_name("EX9-012 Inherited: +4000 DP")
        effect7.set_effect_description("[Your Turn] This Digimon gets +4000 DP.")
        effect7.is_inherited_effect = True
        effect7.dp_modifier = 4000

        def condition7(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect7.set_can_use_condition(condition7)
        effects.append(effect7)

        return effects
