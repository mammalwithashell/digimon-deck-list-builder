from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_093(CardScript):
    """BT21-093 Raging Serpentine"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.BeforePayCost
        # When this card would be used, if your opponent has 3 or fewer security cards, reduce the use cost by 
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("BT21-093 Reduce Play Cost -4")
        effect0.set_effect_description("When this card would be used, if your opponent has 3 or fewer security cards, reduce the use cost by ")
        effect0.set_hash_string("PlayCost-4_BT21_093")
        effect0.cost_reduction = 4

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Only reduce cost if opponent has 3 or fewer security cards
            if context.get('card_source') is not card:
                return False
            player = card.owner if card else None
            if player:
                enemy = player.enemy if player else None
                if enemy and len(enemy.security_cards) > 3:
                    return False
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] Delete 1 of your opponent's highest DP Digimon. Then, place this card in the battle area.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OptionSkill)
        effect2.set_effect_name("BT21-093 Delete 1 of your opponent's Digimon with the highest DP")
        effect2.set_effect_description("[Main] Delete 1 of your opponent's highest DP Digimon. Then, place this card in the battle area.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Delete opponent's highest DP Digimon"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            dps = [p.dp or 0 for p in enemy.battle_area if p.is_digimon]
            max_dp = max(dps) if dps else 0
            def target_filter(p):
                return p.is_digimon and (p.dp or 0) >= max_dp
            def on_delete(target_perm):
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Factory effect: delay
        # Delay
        effect3 = ICardEffect()
        effect3.set_effect_name("BT21-093 Delay")
        effect3.set_effect_description("Delay")
        effect3._is_delay = True

        def condition3(context: Dict[str, Any]) -> bool:
            # Delay effect — the option card must be on field (placed via Delay).
            # The Reptile/Dragonkin trait check is on the digivolve targets,
            # not on this option card.
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnLoseSecurity
        # [All Turns] When your opponent's security stack is removed from, <Delay> (After this card is placed, by trashing it the next turn or later, activate the effect below).\r\n・1 of your [Reptile]/[Dragonkin] trait Digimon may digivolve into a [Reptile]/[Dragonkin] trait Digimon card in the hand without paying the cost.\r\n
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnLoseSecurity)
        effect4.set_effect_name("BT21-093 1 of your [Reptile]/[Dragonkin] trait Digimon may digivolve")
        effect4.set_effect_description("[All Turns] When your opponent's security stack is removed from, <Delay> (After this card is placed, by trashing it the next turn or later, activate the effect below).\\r\\n・1 of your [Reptile]/[Dragonkin] trait Digimon may digivolve into a [Reptile]/[Dragonkin] trait Digimon card in the hand without paying the cost.\\r\\n")
        effect4.is_optional = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be opponent's security, not own — use event_player (who lost security)
            event_player = context.get('event_player') or context.get('player')
            if event_player and card and card.owner:
                # The player who lost security must be the opponent, not the card owner
                if event_player is card.owner:
                    return False
            # Delay turn check: cannot activate on the placing turn
            perm = card.permanent_of_this_card()
            game = context.get('game')
            if perm and game and perm.turn_played >= game.turn_count:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Select own Reptile/Dragonkin Digimon, digivolve from hand free"""
            # Use card.owner (not ctx player) — ctx player is set by engine to perm owner,
            # but for reactive triggers the digivolve targets the card OWNER's Digimon
            player = card.owner if card else ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Trash the option card from battle area (delay cost)
            perm = card.permanent_of_this_card() if card else None
            if perm and perm in player.battle_area:
                player.battle_area.remove(perm)
                for source in perm.card_sources:
                    player.trash_cards.append(source)
            # Select own Reptile/Dragonkin Digimon to digivolve
            def own_filter(p):
                if not p.is_digimon:
                    return False
                traits = getattr(p.top_card, 'card_traits', []) or []
                return any('Reptile' in t or 'Dragonkin' in t for t in traits)
            def on_selected(target_perm):
                # Digivolve into Reptile/Dragonkin from hand without paying cost
                def digi_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    traits = getattr(c, 'card_traits', []) or []
                    return any('Reptile' in t or 'Dragonkin' in t for t in traits)
                game.effect_digivolve_from_hand(
                    player, target_perm, digi_filter, cost_override=0, is_optional=True)
            game.effect_select_own_permanent(
                player, on_selected, filter_fn=own_filter, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Delete 1 of your opponent's Digimon with the highest DP.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.SecuritySkill)
        effect5.set_effect_name("BT21-093 Delete 1 of your opponent's Digimon with the highest DP")
        effect5.set_effect_description("[Security] Delete 1 of your opponent's Digimon with the highest DP.")
        effect5.is_security_effect = True

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Delete opponent's highest DP Digimon (security)"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            dps = [p.dp or 0 for p in enemy.battle_area if p.is_digimon]
            max_dp = max(dps) if dps else 0
            def target_filter(p):
                return p.is_digimon and (p.dp or 0) >= max_dp
            def on_delete(target_perm):
                if enemy:
                    enemy.delete_permanent(target_perm)
            game.effect_select_opponent_permanent(
                player, on_delete, filter_fn=target_filter, is_optional=False)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
