from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class ST21_13(CardScript):
    """ST21-13 Matt Ishida & T.K. Takaishi | Tamer | White/Yellow | Cost 3

    [Your Turn] When any Digimon cards with the [ADVENTURE] trait would be
    played from your hand, by suspending this Tamer, reduce the play cost by 1.

    [Your Turn] All of your level 5 or higher Digimon with the [ADVENTURE]
    trait gain <Rush>.

    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Helper: check if a permanent is an ADVENTURE Digimon with level >= 5
        def _is_adventure_lv5_plus(perm) -> bool:
            if not getattr(perm, 'is_digimon', False):
                return False
            level = getattr(perm, 'level', None)
            if level is None or level < 5:
                return False
            top = getattr(perm, 'top_card', None)
            if not top:
                return False
            traits = getattr(top, 'card_traits', []) or []
            return any('ADVENTURE' in t for t in traits)

        # --- Effect 0: Cost reduction ---
        # [Your Turn] When any Digimon cards with the [ADVENTURE] trait would
        # be played from your hand, by suspending this Tamer, reduce the play
        # cost by 1.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.BeforePayCost)
        effect0.set_effect_name("ST21-13 Reduce ADVENTURE Digimon play cost by 1")
        effect0.set_effect_description(
            "[Your Turn] When any Digimon cards with the [ADVENTURE] trait "
            "would be played from your hand, by suspending this Tamer, reduce "
            "the play cost by 1."
        )
        effect0.is_optional = True
        effect0.cost_reduction = 1

        def condition0(context: Dict[str, Any]) -> bool:
            # Must be on field
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be owner's turn
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return False
            # Tamer must not be suspended
            own_perm = card.permanent_of_this_card()
            if own_perm and own_perm.is_suspended:
                return False
            # Leak guard: cannot reduce its own play cost
            cs = context.get('card_source')
            if cs is card:
                return False
            # Must be same owner
            if cs and getattr(cs, 'owner', None) is not owner:
                return False
            # Card being played must be a Digimon
            if not getattr(cs, 'is_digimon', False):
                return False
            # Card being played must have ADVENTURE trait
            traits = getattr(cs, 'card_traits', []) or []
            if not any('ADVENTURE' in t for t in traits):
                return False
            # Source zone must be hand (default)
            source_zone = context.get('source_zone', 'hand')
            if source_zone != 'hand':
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Suspend this Tamer as the cost for the reduction."""
            own_perm = card.permanent_of_this_card() if card else None
            if own_perm and not own_perm.is_suspended:
                own_perm.suspend()

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Rush aura ---
        # [Your Turn] All of your level 5 or higher Digimon with the [ADVENTURE]
        # trait gain <Rush>.
        # Uses _applies_to_all_own_digimon aura pattern so has_keyword scans
        # this effect on other permanents.
        effect1 = ICardEffect()
        effect1.set_effect_name("ST21-13 [Your Turn] ADVENTURE Lv.5+ Digimon gain Rush")
        effect1.set_effect_description(
            "[Your Turn] All of your level 5 or higher Digimon with the "
            "[ADVENTURE] trait gain <Rush>."
        )
        effect1._is_rush = True
        effect1._applies_to_all_own_digimon = True
        effect1._keyword_permanent_condition = _is_adventure_lv5_plus

        def condition1(context: Dict[str, Any]) -> bool:
            # Must be on field
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be owner's turn
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return False
            # When checked for a specific permanent via has_keyword,
            # context['permanent'] is the permanent being queried
            ctx_perm = context.get('permanent')
            if ctx_perm is not None:
                return _is_adventure_lv5_plus(ctx_perm)
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: Security ---
        # [Security] Play this card without paying the cost.
        effect2 = ICardEffect()
        effect2.set_effect_name("ST21-13 Security: Play this card")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
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
