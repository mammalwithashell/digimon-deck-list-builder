from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT20_083(CardScript):
    """BT20-083 Omekamon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []
        card.also_treated_as_names = ['X Antibody']

        # Timing: EffectTiming.None
        # Also Treated As
        effect0 = ICardEffect()
        effect0.set_effect_name("BT20-083 Also treated as [X Antibody]")
        effect0.set_effect_description("Also Treated As")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Also Treated As"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Also treated as [Name] — name aliasing not modeled in engine
            pass  # descriptive-tagged: also_treated_as_name

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Factory effect: blocker
        # Blocker
        effect1 = ICardEffect()
        effect1.set_effect_name("BT20-083 Blocker")
        effect1.set_effect_description("Blocker")
        effect1._is_blocker = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] If you have 1 or fewer security cards, this Digimon may digivolve into [Omnimon (X Antibody)] in the hand, ignoring digivolution requirements and without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT20-083 Digivolve this Digimon into [Omnimon (X Antibody)]")
        effect2.set_effect_description("[On Play] If you have 1 or fewer security cards, this Digimon may digivolve into [Omnimon (X Antibody)] in the hand, ignoring digivolution requirements and without paying the cost.")
        effect2.is_on_play = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Condition: 1 or fewer security cards
            player = card.owner if card else None
            if not player or len(player.security_cards) > 1:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Digivolve into Omnimon (X Antibody)"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            def digi_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not (any('Omnimon (X Antibody)' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_digivolve_from_hand(
                player, perm, digi_filter, cost_override=0,
                ignore_requirements=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may place this card as the bottom digivolution card of your [King Drasil_7D6] in the breeding area.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDestroyedAnyone)
        effect3.set_effect_name("BT20-083 Place under [King Drasil_7D6] in the breeding area.")
        effect3.set_effect_description("[On Deletion] You may place this card as the bottom digivolution card of your [King Drasil_7D6] in the breeding area.")
        effect3.is_optional = True
        effect3.is_on_deletion = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Check that the player has a King Drasil_7D6 in the BREEDING AREA
            player = card.owner if card else None
            if not player or not player.breeding_area:
                return False
            breeding = player.breeding_area
            if not breeding.contains_card_name('King Drasil_7D6'):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Place this card as the bottom digivolution card of King Drasil_7D6 in breeding area."""
            player = ctx.get('player')
            if not player or not player.breeding_area:
                return
            breeding = player.breeding_area
            if not breeding.contains_card_name('King Drasil_7D6'):
                return
            # Remove card from trash (it was just deleted and sent to trash)
            if card in player.trash_cards:
                player.trash_cards.remove(card)
            # Place this card at bottom of breeding digi stack
            breeding.card_sources.insert(0, card)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnLoseSecurity
        # [Breeding] [Opponent's Turn] When your security stack is removed from, by suspending this Digimon, play 1 [Omekamon] from this Digimon's digivolution cards without paying the cost.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnLoseSecurity)
        effect4.set_effect_name("BT20-083 play 1 [Omekamon] from digivolution cards")
        effect4.set_effect_description("[Breeding] [Opponent's Turn] When your security stack is removed from, by suspending this Digimon, play 1 [Omekamon] from this Digimon's digivolution cards without paying the cost.")
        effect4.is_inherited_effect = True
        effect4.is_optional = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # [Opponent's Turn] — only on opponent's turn
            player = card.owner if card else None
            if not player or player.is_my_turn:
                return False
            # Must not already be suspended (cost is "by suspending this Digimon")
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if not permanent or permanent.is_suspended:
                return False
            # Must have an Omekamon in digivolution cards
            digi_sources = permanent.card_sources[:-1] if len(permanent.card_sources) > 1 else []
            has_omekamon = any(
                any('Omekamon' in _n for _n in getattr(src, 'card_names', []))
                for src in digi_sources
            )
            if not has_omekamon:
                return False
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Cost: suspend this Digimon. Action: Play 1 Omekamon from digivolution cards."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            # Cost: suspend self
            perm.suspend()
            # Find Omekamon in digivolution cards (card_sources[:-1])
            digi_sources = list(perm.card_sources[:-1]) if len(perm.card_sources) > 1 else []
            for source in digi_sources:
                if any('Omekamon' in _n for _n in getattr(source, 'card_names', [])):
                    # Remove from digivolution cards
                    perm.card_sources.remove(source)
                    # Play without paying cost
                    played_perm = player.play_card_from_source(source, pay_cost=False)
                    if played_perm and hasattr(game, 'execute_effects'):
                        game.execute_effects(
                            EffectTiming.OnEnterFieldAnyone,
                            {"played_card": source, "played_permanent": played_perm,
                             "event_player": player},
                        )
                    break

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
