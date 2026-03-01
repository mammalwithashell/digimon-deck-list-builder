from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_084(CardScript):
    """BT23-084 Erika Mishima"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Start of Your Main Phase] If you have a Digimon with the [CS] trait, gain 1 memory.
        effect_mem = ICardEffect()
        effect_mem.set_timing(EffectTiming.OnStartMainPhase)
        effect_mem.set_effect_name("BT23-084 Gain 1 memory")
        effect_mem.set_effect_description("[Start of Your Main Phase] If you have a Digimon with the [CS] trait, gain 1 memory.")

        def condition_mem(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            player = card.owner if card else None
            if player:
                has_cs = any(
                    p.is_digimon and any('CS' in t for t in (getattr(p.top_card, 'card_traits', []) or []))
                    for p in player.battle_area
                )
                if not has_cs:
                    return False
            return True
        effect_mem.set_can_use_condition(condition_mem)

        def process_mem(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.add_memory(1)
        effect_mem.set_on_process_callback(process_mem)
        effects.append(effect_mem)

        # [End of Your Turn] By suspending this Tamer and returning 1 of your [Hudie] Digimon to hand,
        # play 1 level 3 [CS] Digimon from hand to empty breeding area free.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEndTurn)
        effect0.set_effect_name("BT23-084 Suspend + bounce Hudie -> play Lv.3 CS to breeding")
        effect0.set_effect_description("[End of Your Turn] By suspending this Tamer and returning 1 of your Digimon with the [Hudie] trait to the hand, you may play 1 level 3 Digimon card with the [CS] trait from your hand to your empty breeding area without paying the cost.")
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm and tamer_perm.is_suspended:
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Suspend this Tamer as cost
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm:
                tamer_perm.suspend()
            # Select own Hudie Digimon to return to hand
            def hudie_filter(p):
                if not p.is_digimon:
                    return False
                traits = getattr(p.top_card, 'card_traits', []) or []
                return any('Hudie' in t for t in traits)
            def on_bounce(target_perm):
                player.bounce_permanent_to_hand(target_perm)
                # Then play Lv.3 CS Digimon from hand to breeding
                def play_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    if (getattr(c, 'level', None) or 0) != 3:
                        return False
                    c_traits = getattr(c, 'card_traits', []) or []
                    return any('CS' in t for t in c_traits)
                game.effect_play_from_zone(
                    player, 'hand', play_filter, free=True, is_optional=True)
            game.effect_select_own_permanent(
                player, on_bounce, filter_fn=hudie_filter, is_optional=False)
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Security: Play this card
        effect1 = ICardEffect()
        effect1.set_effect_name("BT23-084 Security: Play this card")
        effect1.set_effect_description("Security: Play this card")
        effect1.is_security_effect = True
        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Inherited: [Your Turn] While this Digimon is Hudiemon/Eater Legion/Eater EDEN, gains Alliance
        effect2 = ICardEffect()
        effect2.set_effect_name("BT23-084 Alliance")
        effect2.set_effect_description("Alliance")
        effect2.is_inherited_effect = True
        effect2.is_on_attack = True
        effect2._is_alliance = True
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Hudiemon') or permanent.contains_card_name('Eater Legion') or permanent.contains_card_name('Eater EDEN'))):
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
