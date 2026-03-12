from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_063(CardScript):
    """EX10-063 Close | Tamer"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Start of Your Main Phase] By returning this Tamer to the bottom of the deck,
        # you may play 1 [Close] from your hand without paying the cost.
        # Then, if you don't have a Digimon, you may play 1 [Sunarizamon] from your trash
        # without paying the cost.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("EX10-063 Return to deck to play Close from hand, then maybe Sunarizamon")
        effect0.set_effect_description(
            "[Start of Your Main Phase] By returning this Tamer to the bottom of the deck, "
            "you may play 1 [Close] from your hand without paying the cost. Then, if you don't "
            "have a Digimon, you may play 1 [Sunarizamon] from your trash without paying the cost."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not owner or not owner.is_my_turn:
                return False
            # Worthwhile if there is a Close in hand or a Sunarizamon in trash when no Digimon
            has_close = any(
                any('Close' in n for n in getattr(c, 'card_names', []))
                for c in owner.hand_cards
            )
            if has_close:
                return True
            has_digimon = any(p.is_digimon for p in owner.battle_area)
            if not has_digimon:
                has_suna = any(
                    any('Sunarizamon' in n for n in getattr(c, 'card_names', []))
                    for c in owner.trash_cards
                )
                return has_suna
            return False

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: return this tamer to the bottom of the deck
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            player.return_permanent_to_deck_bottom(tamer_perm)
            # Effect: play 1 [Close] from hand without paying the cost
            def close_filter(c):
                return any('Close' in n for n in getattr(c, 'card_names', []))
            game.effect_play_from_zone(
                player, 'hand', close_filter, free=True, is_optional=True,
                prompt="Play 1 [Close] from your hand without paying the cost."
            )
            # Then: if you don't have a Digimon, play 1 [Sunarizamon] from trash free
            has_digimon = any(p.is_digimon for p in player.battle_area)
            if not has_digimon:
                def suna_filter(c):
                    return any('Sunarizamon' in n for n in getattr(c, 'card_names', []))
                game.effect_play_from_zone(
                    player, 'trash', suna_filter, free=True, is_optional=True,
                    prompt="Play 1 [Sunarizamon] from your trash without paying the cost."
                )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [All Turns] When effects trash any of your [Mineral] or [Rock] trait Digimon's
        # digivolution cards, by suspending this Tamer, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnDigivolutionCardDiscarded)
        effect1.set_effect_name("EX10-063 Suspend tamer to gain 1 memory when Mineral/Rock digi-card trashed")
        effect1.set_effect_description(
            "[All Turns] When effects trash any of your [Mineral] or [Rock] trait Digimon's "
            "digivolution cards, by suspending this Tamer, gain 1 memory."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Tamer must not already be suspended (it will be suspended as cost)
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and tamer_perm.is_suspended:
                return False
            # The permanent whose digivolution card was trashed must belong to this player
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
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if not player:
                return
            # Cost: suspend this tamer
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            tamer_perm.suspend()
            # Effect: gain 1 memory
            player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Security: Play this card without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("EX10-063 Security: Play this card")
        effect2.set_effect_description("Security: Play this card without paying the cost.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
