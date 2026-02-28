from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT11_092(CardScript):
    """BT11-092 Analogman"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] By trashing 1 level 5 card with [Cyborg] in its traits in your hand, gain 1 memory and <Draw 1>.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT11-092 Trash 1 card from hand to gain Memory +1 and Draw 1")
        effect0.set_effect_description("[Start of Your Main Phase] By trashing 1 level 5 card with [Cyborg] in its traits in your hand, gain 1 memory and <Draw 1>.")
        effect0.is_optional = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Draw 1, Gain 1 memory, Trash From Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def hand_filter(c):
                if not (any('Cyborg' in _t for _t in (getattr(c, 'card_traits', []) or []))):
                    return False
                return True
            def on_trashed(selected):
                if selected in player.hand_cards:
                    player.hand_cards.remove(selected)
                    player.trash_cards.append(selected)
            game.effect_select_hand_card(
                player, hand_filter, on_trashed, is_optional=True)
            if player:
                player.draw_cards(1)
            if player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OnAllyAttack
        # [Opponent's Turn] When an opponent's Digimon attacks a player, by suspending this Tamer, switch the target of attack to 1 of your level 6 Digimon with [Machine] in its traits.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT11-092 Switch Attack Target to your Digimon")
        effect1.set_effect_description("[Opponent's Turn] When an opponent's Digimon attacks a player, by suspending this Tamer, switch the target of attack to 1 of your level 6 Digimon with [Machine] in its traits.")
        effect1.is_optional = True
        effect1.is_on_attack = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend, Redirect Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                if not (any('Machine' in t for t in (getattr(p.top_card, 'card_traits', []) or []))):
                    return False
                return True
            def on_suspend(target_perm):
                target_perm.suspend()
            game.effect_select_opponent_permanent(
                player, on_suspend, filter_fn=target_filter, is_optional=True)
            # Redirect attack target (SwitchDefender) — not yet in engine
            pass  # descriptive-tagged: redirect_attack

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect2 = ICardEffect()
        effect2.set_effect_name("BT11-092 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
