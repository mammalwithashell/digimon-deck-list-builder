from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX1_065(CardScript):
    """EX1-065 Diaboromon | Lv.6 (White, Cost 12)

    [Security] At the end of the battle, you may play 1 [Diaboromon] Token
        without paying the cost.
    [Opponent's Turn] All of your [Diaboromon] gain <Blocker>.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Security] Play Diaboromon Token at end of battle ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.SecuritySkill)
        effect0.set_effect_name("EX1-065 Security: Play Diaboromon Token")
        effect0.set_effect_description(
            "[Security] At the end of the battle, you may play 1 [Diaboromon] "
            "Token without paying the cost."
        )
        effect0.is_security_effect = True
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                game.effect_play_token(player, 'diaboromon')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: [Opponent's Turn] All Diaboromon gain Blocker ---
        # This is an aura effect that grants Blocker to all Diaboromon you control.
        # We model it by granting GRANT_BLOCKER modifier to all own Diaboromon.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX1-065 All Diaboromon gain Blocker")
        effect1.set_effect_description(
            "[Opponent's Turn] All of your [Diaboromon] gain <Blocker>."
        )
        effect1.is_declarative = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner:
                return False
            # Must be opponent's turn
            if owner.is_my_turn:
                return False
            return True
        effect1.set_can_use_condition(condition1)

        # To actually grant Blocker, we need to use the modifier system
        # or grant keyword. The cleanest is to use a timing hook.
        # But since this is "All Turns" aura, let's register modifiers
        # when this card enters the field.

        effects.append(effect1)

        # --- Effect 2: Register Blocker grant on enter field ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX1-065 Register Blocker aura")
        effect2.set_effect_description(
            "Internal: Register GRANT_BLOCKER for all Diaboromon on opponent's turn."
        )
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            own_perm = card.permanent_of_this_card() if card else None
            if not own_perm:
                return
            from engine_py_legacy.engine.interfaces.modifiers import ModifierType
            game.register_modifier(
                own_perm,
                ModifierType.GRANT_BLOCKER,
                condition=lambda target, ctx: (
                    target.is_digimon
                    and target.contains_card_name('Diaboromon')
                    and target.top_card
                    and target.top_card.owner
                    and not target.top_card.owner.is_my_turn
                ),
                expiry='permanent',
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Also register when digivolving
        effect2b = ICardEffect()
        effect2b.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2b.set_effect_name("EX1-065 Register Blocker aura (digivolve)")
        effect2b.set_effect_description("Internal: Register GRANT_BLOCKER on digivolve.")
        effect2b.is_when_digivolving = True

        def condition2b(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2b.set_can_use_condition(condition2b)
        effect2b.set_on_process_callback(process2)
        effects.append(effect2b)

        return effects
