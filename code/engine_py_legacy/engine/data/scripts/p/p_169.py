from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_169(CardScript):
    """P-169 Close | Tamer"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("P-169 Gain 1 memory if opponent has a Digimon")
        effect0.set_effect_description("[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner or not owner.is_my_turn:
                return False
            # Condition: opponent has at least 1 Digimon
            enemy = owner.enemy
            if enemy is None:
                return False
            return any(p.is_digimon for p in enemy.battle_area)

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [All Turns] When effects trash digivolution cards of any of your [Mineral] or [Rock]
        # trait Digimon, by suspending this Tamer, place 1 card with the [Mineral] or [Rock]
        # trait from your trash as any of your Digimon's bottom digivolution card.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect1.set_effect_name("P-169 Suspend tamer to place 1 Mineral/Rock from trash as bottom digi-source")
        effect1.set_effect_description(
            "[All Turns] When effects trash digivolution cards of any of your [Mineral] or [Rock] "
            "trait Digimon, by suspending this Tamer, place 1 card with the [Mineral] or [Rock] "
            "trait from your trash as any of your Digimon's bottom digivolution card."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Tamer must not already be suspended (it will be suspended as cost)
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and tamer_perm.is_suspended:
                return False
            # The permanent whose digi-card was trashed must belong to this player
            owner = card.owner if card else None
            if owner is None:
                return False
            event_perm = context.get('event_permanent')
            if event_perm is None:
                return False
            if event_perm not in owner.battle_area:
                return False
            # The permanent must have [Mineral] or [Rock] trait
            if not (event_perm.has_trait('Mineral') or event_perm.has_trait('Rock')):
                return False
            # Must have a Mineral/Rock card in trash to place
            if not any(
                'Mineral' in getattr(c, 'card_traits', []) or 'Rock' in getattr(c, 'card_traits', [])
                for c in owner.trash_cards
            ):
                return False
            # Must have at least one Digimon on the field to attach to
            if not any(p.is_digimon for p in owner.battle_area):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: suspend this tamer
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            tamer_perm.suspend()

            # Player selects which Mineral/Rock card from trash to place
            def source_filter(source_card) -> bool:
                traits = getattr(source_card, 'card_traits', [])
                return 'Mineral' in traits or 'Rock' in traits

            from ....game.constants import SEL_TRASH_START
            from ....data.enums import GamePhase

            valid = []
            for i, c in enumerate(player.trash_cards):
                if source_filter(c):
                    valid.append(SEL_TRASH_START + i)
            if not valid:
                return

            def on_trash_selected(action_id: int):
                idx = action_id - SEL_TRASH_START
                if not (0 <= idx < len(player.trash_cards)):
                    return
                source_to_place = player.trash_cards[idx]
                player.trash_cards.remove(source_to_place)

                def target_filter(p):
                    return p.is_digimon

                def on_target(target_perm):
                    target_perm.add_card_source_bottom(source_to_place)

                game.effect_select_own_permanent(
                    player, on_target, filter_fn=target_filter, is_optional=False,
                    prompt="Select 1 of your Digimon to place a [Mineral] or [Rock] card from trash under."
                )

            game.request_selection(
                GamePhase.SelectTrash, player, on_trash_selected, valid,
                is_optional=False,
                prompt="Select 1 card with [Mineral] or [Rock] trait from your trash to place as a bottom digivolution card."
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Security: Play this card without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("P-169 Security: Play this card")
        effect2.set_effect_description("Security: Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
