from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT5_093(CardScript):
    """BT5-093 Tai Kamiya & Matt Ishida | Tamer | White | Cost 4

    [Start of Your Turn] If your opponent has a level 6 or higher Digimon
        in play, gain 2 memory.
    [Your Turn] All of your [Omnimon] gain <Security Attack +1>.
    Security Effect: [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Start of Your Turn] Gain 2 memory ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartTurn)
        effect0.set_effect_name("BT5-093 Start of Turn: Gain 2 memory")
        effect0.set_effect_description(
            "[Start of Your Turn] If your opponent has a level 6 or higher "
            "Digimon in play, gain 2 memory."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Check opponent has a Lv.6+ Digimon
            enemy = card.owner.enemy if card.owner else None
            if not enemy:
                return False
            for p in enemy.battle_area:
                if p.is_digimon and hasattr(p, 'level') and p.level is not None and p.level >= 6:
                    return True
            return False
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Gain 2 memory."""
            game = ctx.get('game')
            if game:
                game.memory += 2
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Your Turn] All [Omnimon] gain <Security Attack +1> ---
        # This is a static/continuous effect. We model it as a declarative
        # effect with _security_attack_modifier that the engine checks.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT5-093 Your Turn: Omnimon SA+1")
        effect1.set_effect_description(
            "[Your Turn] All of your [Omnimon] gain <Security Attack +1>."
        )
        effect1._security_attack_modifier = 1

        def sa_permanent_condition(permanent) -> bool:
            """Only applies to own [Omnimon] Digimon during your turn."""
            if not permanent.is_digimon:
                return False
            if not permanent.contains_card_name('Omnimon'):
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            if permanent.top_card and permanent.top_card.owner is not owner:
                return False
            return True
        effect1._sa_permanent_condition = sa_permanent_condition

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [Security] Play this card without paying the cost ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("BT5-093 Security: Play free")
        effect2.set_effect_description("[Security] Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play this card from security without paying the cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game and card:
                player.play_card_from_source(card, pay_cost=False)
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
