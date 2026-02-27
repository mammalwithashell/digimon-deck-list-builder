from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_086(CardScript):
    """BT14-086 Satsuki Tamahime"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: security_play
        # Security: Play this card
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-086 Security: Play this card")
        effect0.set_effect_description("Security: Play this card")
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnStartMainPhase
        # [Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-086 Memory +1")
        effect1.set_effect_description("[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.")

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            opponent = getattr(card.owner, 'opponent', None)
            if not opponent:
                return False
            opponent_battle_area = getattr(opponent, 'battle_area', None) or []
            return len(opponent_battle_area) > 0

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnDeclaration
        # [Main] <Mind Link> with 1 of your Digimon with [Numemon] or [Monzaemon] in its name, or the [DigiPolice] trait. (Place this Tamer as that Digimon's bottom digivolution card if there are no Tamer cards in its digivolution cards.)
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-086 Mind Link")
        effect2.set_effect_description("[Main] <Mind Link> with 1 of your Digimon with [Numemon] or [Monzaemon] in its name, or the [DigiPolice] trait. (Place this Tamer as that Digimon's bottom digivolution card if there are no Tamer cards in its digivolution cards.)")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Mind Link"""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_link_to_permanent(player, card, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        def linked_target_matches() -> bool:
            perm = card.permanent_of_this_card() if card else None
            if perm is None:
                return False
            linked = getattr(perm, 'mind_linked_perm', None)
            if linked is None:
                return False
            name = (getattr(linked, 'name', '') or '').lower()
            if 'numemon' in name or 'monzaemon' in name:
                return True
            traits = getattr(linked, 'traits', None) or []
            for t in traits:
                if str(t).lower() == 'digipolice':
                    return True
            return False

        # Factory effect: jamming
        # Jamming
        effect3 = ICardEffect()
        effect3.set_effect_name("BT14-086 Jamming")
        effect3.set_effect_description("Jamming")
        effect3.is_inherited_effect = True
        effect3._is_jamming = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return linked_target_matches()
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Factory effect: reboot
        # Reboot
        effect4 = ICardEffect()
        effect4.set_effect_name("BT14-086 Reboot")
        effect4.set_effect_description("Reboot")
        effect4.is_inherited_effect = True
        effect4._is_reboot = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return linked_target_matches()
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEndTurn
        # [End of All Turns] You may play 1 [Satsuki Tamahime] from this Digimon's digivolution cards without paying the cost.
        effect5 = ICardEffect()
        effect5.set_effect_name("BT14-086 Play 1 [Satsuki Tamahime] from this Digimon's digivolution cards")
        effect5.set_effect_description("[End of All Turns] You may play 1 [Satsuki Tamahime] from this Digimon's digivolution cards without paying the cost.")
        effect5.is_inherited_effect = True
        effect5.is_optional = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            game = ctx.get('game')
            permanent = ctx.get('permanent')
            if not (player and game and permanent):
                return

            sources = getattr(permanent, 'digivolution_cards', None) or []
            target = None
            for c in sources:
                cid = getattr(c, 'card_id', '') or ''
                cname = getattr(c, 'name', '') or ''
                if cid == 'BT14-086' or cname == 'Satsuki Tamahime':
                    target = c
                    break
            if target is None:
                return
            game.play_card(player, target, free=True)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
