from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT12_050(CardScript):
    """BT12-050 Stingmon | Lv.4 Digimon"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Your Turn] When this Digimon would DNA digivolve into a blue Digimon
        # card, gain 1 memory.
        effect_dna = ICardEffect()
        effect_dna.set_timing(EffectTiming.WhenDigivolving)
        effect_dna.set_effect_name("BT12-050 DNA digivolve memory +1")
        effect_dna.set_effect_description(
            "[Your Turn] When this Digimon would DNA digivolve into a blue "
            "Digimon card, gain 1 memory."
        )
        effect_dna.set_hash_string("Memory+1_BT12_050")

        def condition_dna(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            if not context.get('is_dna_digivolve'):
                return False
            perm = card.permanent_of_this_card()
            if not perm:
                return False
            digivolved = context.get('digivolved_permanent')
            if not digivolved:
                return False
            if card not in digivolved.card_sources:
                return False
            top = digivolved.top_card
            if top is None:
                return False
            from ....data.enums import CardColor
            colors = getattr(top, 'card_colors', []) or []
            return CardColor.Blue in colors

        effect_dna.set_can_use_condition(condition_dna)

        def process_dna(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                player.add_memory(1)
                game.logger.log(
                    f"[BT12-050] {player.player_name} gained 1 memory "
                    f"(DNA digivolve into blue)."
                )

        effect_dna.set_on_process_callback(process_dna)
        effects.append(effect_dna)

        # [Inherited] [Your Turn] While this Digimon has [Imperialdramon] in its name
        # or the [Free] trait, it gains <Piercing>.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT12-050 Piercing (inherited)")
        effect0.set_effect_description(
            "[Your Turn] While this Digimon has [Imperialdramon] in its name or the "
            "[Free] trait, it gains <Piercing>."
        )
        effect0._is_piercing = True
        effect0.is_inherited_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            top = perm.top_card
            if top is None:
                return False
            if perm.contains_card_name('Imperialdramon'):
                return True
            if any('Free' in tr for tr in getattr(top, 'card_traits', [])):
                return True
            return False

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        return effects
