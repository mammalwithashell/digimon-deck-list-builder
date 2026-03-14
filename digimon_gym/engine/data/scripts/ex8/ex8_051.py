from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX8_051(CardScript):
    """EX8-051 Proganomon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Keyword: Collision
        effect_collision = ICardEffect()
        effect_collision.set_effect_name("EX8-051 Collision")
        effect_collision.set_effect_description(
            "<Collision> (During this Digimon's attack, all of your opponent's Digimon gain "
            "<Blocker>, and must block if possible.)"
        )
        effect_collision._is_collision = True

        def condition_collision(context: Dict[str, Any]) -> bool:
            return True
        effect_collision.set_can_use_condition(condition_collision)
        effects.append(effect_collision)

        # Keyword: Piercing
        effect_piercing = ICardEffect()
        effect_piercing.set_effect_name("EX8-051 Piercing")
        effect_piercing.set_effect_description(
            "<Piercing> (When this Digimon attacks and deletes an opponent's Digimon and "
            "survives the battle, it performs any security checks it normally would.)"
        )
        effect_piercing._is_piercing = True

        def condition_piercing(context: Dict[str, Any]) -> bool:
            return True
        effect_piercing.set_can_use_condition(condition_piercing)
        effects.append(effect_piercing)

        # Keyword: Fragment (3)
        effect_fragment = ICardEffect()
        effect_fragment.set_effect_name("EX8-051 Fragment")
        effect_fragment.set_effect_description(
            "<Fragment (3)> (When this Digimon would be deleted, by trashing any 3 of its "
            "digivolution cards, it isn't deleted.)"
        )
        effect_fragment._is_fragment = True

        def condition_fragment(context: Dict[str, Any]) -> bool:
            return True
        effect_fragment.set_can_use_condition(condition_fragment)
        effects.append(effect_fragment)

        # Inherited: When effects trash this card from a [Mineral] or [Rock] trait
        # Digimon's digivolution cards, <De-Digivolve 1> 1 of your opponent's Digimon.
        effect_inh = ICardEffect()
        effect_inh.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect_inh.set_effect_name("EX8-051 inherited De-Digivolve 1")
        effect_inh.set_effect_description(
            "When effects trash this card from a [Mineral] or [Rock] trait Digimon's "
            "digivolution cards, <De-Digivolve 1> 1 of your opponent's Digimon."
        )
        effect_inh.is_inherited_effect = True

        def condition_inh(context: Dict[str, Any]) -> bool:
            # Check this card was the one trashed
            trashed_cards = context.get('trashed_cards', [])
            if card not in trashed_cards:
                return False
            # Check the permanent has [Mineral] or [Rock] trait
            permanent = context.get('permanent')
            if permanent is None:
                return False
            if not (permanent.has_trait('Mineral') or permanent.has_trait('Rock')):
                return False
            return True

        effect_inh.set_can_use_condition(condition_inh)

        def process_inh(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)

            game.effect_select_opponent_permanent(
                player, on_de_digivolve,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False
            )

        effect_inh.set_on_process_callback(process_inh)
        effects.append(effect_inh)

        return effects
