from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_086(CardScript):
    """BT24-086 The Crossroad Witch"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []
        card.also_treated_as_names = ['Shuu Yulin']

        # Timing: EffectTiming.None
        # Also Treated As
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-086 Also treated as [Shuu Yulin]")
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

        # Factory effect: security_play
        # Security: Play this card
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-086 Security: Play this card")
        effect1.set_effect_description("Security: Play this card")
        effect1.is_security_effect = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Play this card from security without paying cost."""
            player = ctx.get('player')
            if player and card:
                player.play_card_from_source(card, pay_cost=False)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnStartMainPhase)
        effect2.set_effect_name("BT24-086 Memory +1")
        effect2.set_effect_description("[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain 1 memory if opponent has a Digimon"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not player:
                return
            enemy = player.enemy
            if enemy and any(p.is_digimon for p in enemy.battle_area):
                player.add_memory(1)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [All Turns] When any of your Digimon are played or digivolve, you may <Mind Link> with 1 of your Digimon with the [X Antibody], [DigiPolice] or [SEEKERS] trait.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("BT24-086 Mind Link")
        effect3.set_effect_description("[All Turns] When any of your Digimon are played or digivolve, you may <Mind Link> with 1 of your Digimon with the [X Antibody], [DigiPolice] or [SEEKERS] trait.")
        effect3.is_optional = True
        effect3.is_on_play = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Mind Link"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_link_to_permanent(player, card, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Factory effect: reboot
        # Reboot
        effect4 = ICardEffect()
        effect4.set_effect_name("BT24-086 Reboot")
        effect4.set_effect_description("Reboot")
        effect4.is_inherited_effect = True
        effect4._is_reboot = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            traits = getattr(permanent.top_card, 'card_traits', []) or [] if permanent and permanent.top_card else []
            if not any('X Antibody' in t or 'DigiPolice' in t or 'SEEKERS' in t for t in traits):
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Factory effect: alliance
        # Alliance
        effect5 = ICardEffect()
        effect5.set_effect_name("BT24-086 Alliance")
        effect5.set_effect_description("Alliance")
        effect5.is_inherited_effect = True
        effect5.is_on_attack = True
        effect5._is_alliance = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            traits = getattr(permanent.top_card, 'card_traits', []) or [] if permanent and permanent.top_card else []
            if not any('X Antibody' in t or 'DigiPolice' in t or 'SEEKERS' in t for t in traits):
                return False
            return True
        effect5.set_can_use_condition(condition5)
        effects.append(effect5)

        # Timing: EffectTiming.OnEndTurn
        # [End of All Turns] You may play 1 [Shuu Yulin] from this Digimon's digivolution cards without paying the cost.
        effect6 = ICardEffect()
        effect6.set_timing(EffectTiming.OnEndTurn)
        effect6.set_effect_name("BT24-086 Play 1 [Shuu Yulin] from this Digimon's digivolution cards")
        effect6.set_effect_description("[End of All Turns] You may play 1 [Shuu Yulin] from this Digimon's digivolution cards without paying the cost.")
        effect6.is_inherited_effect = True
        effect6.is_optional = True

        effect = effect6  # alias for condition closure
        def condition6(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect6.set_can_use_condition(condition6)

        def process6(ctx: Dict[str, Any]):
            """Action: Play 1 [Shuu Yulin] from this Digimon's digivolution cards"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game and perm):
                return
            # Find [Shuu Yulin] in this permanent's digivolution cards
            candidates = [
                cs for cs in perm.card_sources
                if cs is not perm.top_card
                and any('Shuu Yulin' in _n for _n in getattr(cs, 'card_names', []))
            ]
            if not candidates:
                return
            from ....data.enums import GamePhase
            from ....game.constants import SOURCES_PER_FIELD
            # Find field index of this permanent
            field_idx = None
            for fi, fp in enumerate(player.battle_area):
                if fp is perm:
                    field_idx = fi
                    break
            if field_idx is None:
                return
            base = 2000 + field_idx * SOURCES_PER_FIELD
            valid = []
            for i, cs in enumerate(perm.card_sources):
                if cs in candidates and (base + i) < 2168:
                    valid.append(base + i)
            if not valid:
                return

            def on_source_selected(action_id):
                idx = action_id - base
                if not (0 <= idx < len(perm.card_sources)):
                    return
                chosen = perm.card_sources[idx]
                perm.card_sources.remove(chosen)
                player.play_card_from_source(chosen, pay_cost=False)

            game.request_selection(
                GamePhase.SelectSource, player, on_source_selected,
                valid, is_optional=True,
                prompt="Select 1 [Shuu Yulin] from this Digimon's digivolution cards to play.")

        effect6.set_on_process_callback(process6)
        effects.append(effect6)

        return effects
