from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_096(CardScript):
    """BT21-096 The Champion Ultimate Fighter!"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.OptionSkill
        # [Main] For the turn, 1 of your [Marcus Damon]s is also treated as a 12000 DP Digimon, can't digivolve and gains <Rush>. Then, that Digimon may attack your opponent's Digimon. Your opponent's unsuspended Digimon can also be attacked with this effect.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT21-096 Can attack unsuspended Digimon")
        effect0.set_effect_description("[Main] For the turn, 1 of your [Marcus Damon]s is also treated as a 12000 DP Digimon, can't digivolve and gains <Rush>. Then, that Digimon may attack your opponent's Digimon. Your opponent's unsuspended Digimon can also be attacked with this effect.")
        effect0._is_rush = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not (permanent and (permanent.contains_card_name('Marcus Damon'))):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain Keyword Rush, Force Attack, Attack Unsuspended"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.grant_keyword('_is_rush')
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack
            # Can attack unsuspended Digimon via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CAN_ATTACK_UNSUSPENDED, perm,
                    value_fn=lambda: True, expiry='persistent')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.SecuritySkill
        # [Security]  You may play 1 [Marcus Damon] from your hand or trash without paying the cost. Then, add this card to the hand.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.SecuritySkill)
        effect1.set_effect_name("BT21-096 Play Card, Add To Hand")
        effect1.set_effect_description("[Security]  You may play 1 [Marcus Damon] from your hand or trash without paying the cost. Then, add this card to the hand.")
        effect1.is_security_effect = True
        effect1.is_security_effect = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Play Card, Add To Hand"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not (any('Marcus Damon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True)
            # Add card to hand (from trash/reveal)
            if player and player.trash_cards:
                card_to_add = player.trash_cards.pop()
                player.hand_cards.append(card_to_add)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        return effects
