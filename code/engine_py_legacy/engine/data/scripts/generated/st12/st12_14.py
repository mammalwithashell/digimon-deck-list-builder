from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST12_14(CardScript):
    """ST12-14 Aus Generics"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] 1 of your Digimon gets +2000 DP for the turn. Then, if you have a Digimon with [Huckmon] in its name or [Royal Knight] in its traits in play, gain 1 memory, and 1 of your Digimon gains <Piercing> for the turn. (When this Digimon attacks and deletes an opponent's Digimon and survives the battle, it performs any security checks it normally would.)
        effect0 = ICardEffect()
        effect0.set_effect_name("ST12-14 Gain 1 memory, DP +2000, Gain Keyword Piercing")
        effect0.set_effect_description("[Main] 1 of your Digimon gets +2000 DP for the turn. Then, if you have a Digimon with [Huckmon] in its name or [Royal Knight] in its traits in play, gain 1 memory, and 1 of your Digimon gains <Piercing> for the turn. (When this Digimon attacks and deletes an opponent's Digimon and survives the battle, it performs any security checks it normally would.)")
        effect0._is_piercing = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory, DP +2000, Gain Keyword Piercing"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)
            if perm:
                perm.change_dp(2000)
            if not (player and game):
                return
            def target_filter(p):
                if not (p.contains_card_name('Huckmon') or any('Royal Knight' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_piercing')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security] Gain 1 memory, and add this card to its owner's hand.
        effect1 = ICardEffect()
        effect1.set_effect_name("ST12-14 Gain 1 memory, Add To Hand")
        effect1.set_effect_description("[Security] Gain 1 memory, and add this card to its owner's hand.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory, Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if player:
                player.add_memory(1)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
