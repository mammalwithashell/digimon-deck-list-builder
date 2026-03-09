from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from .....core.card_script import CardScript
from .....interfaces.card_effect import ICardEffect
from .....data.enums import EffectTiming

if TYPE_CHECKING:
    from .....core.card_source import CardSource


class ST18_14(CardScript):
    """ST18-14 Shoto Kazama (Tamer)"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Start of Your Turn] If you have 2 or less memory, set it to 3.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("ST18-14 Set memory to 3")
        effect0.set_effect_description(
            "[Start of Your Turn] If your memory is at 2 or less, it becomes 3."
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

        # [Your Turn] When one of your Digimon attacks your opponent's Digimon,
        # by suspending this Tamer, you may change the attack target to another
        # of your opponent's Digimon or the player.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnAllyAttack)
        effect1.set_effect_name("ST18-14 Suspend Tamer to redirect attack")
        effect1.set_effect_description(
            "[Your Turn] When one of your Digimon attacks your opponent's "
            "Digimon, by suspending this Tamer, you may change the attack "
            "target to another of your opponent's Digimon or the player."
        )
        effect1.is_optional = True
        effect1.is_on_attack = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm and perm.is_suspended:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            # Cost: suspend this Tamer
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and not tamer_perm.is_suspended:
                tamer_perm.suspend()
                # Redirect attack target — not yet in engine
                pass  # descriptive-tagged: redirect_attack

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Security Effect [Security] Play this card without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("ST18-14 Security: Play this card")
        effect2.set_effect_description("Security: Play this card")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
