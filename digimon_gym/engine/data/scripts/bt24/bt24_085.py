from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_085(CardScript):
    """BT24-085 Dan Yuki & Kanan Yuki | Tamer"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Start of Your Main Phase] If you have 4 or less memory, gain 1 memory.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("BT24-085 If 4 or less memory, gain 1 memory")
        effect0.set_effect_description(
            "[Start of Your Main Phase] If you have 4 or less memory, gain 1 memory."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Only gain memory if player has 4 or less
            if game.memory <= 4:
                player.add_memory(1)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [End of Your Turn] By suspending this Tamer, you may use 1 [TS] trait Option card
        # with as high or lower a use cost as your opponent's number of Digimon from your hand
        # without paying the cost. Then, 1 of your Digimon with the [TS] trait may attack.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndTurn)
        effect1.set_effect_name(
            "BT24-085 Suspend to use a [TS] Option costing <= opp Digimon count; "
            "then 1 [TS] Digimon may attack"
        )
        effect1.set_effect_description(
            "[End of Your Turn] By suspending this Tamer, you may use 1 [TS] trait Option card "
            "with as high or lower a use cost as your opponent's number of Digimon from your hand "
            "without paying the cost. Then, 1 of your Digimon with the [TS] trait may attack."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Cannot pay cost if already suspended
            tamer_perm = card.permanent_of_this_card() if card else None
            if tamer_perm and tamer_perm.is_suspended:
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

            if not tamer_perm.is_suspended:
                return  # Suspension failed

            # Card text: "use cost as high or lower as your opponent's number of Digimon"
            enemy = player.enemy
            if not enemy:
                return
            opp_digi_count = len([p for p in enemy.battle_area if p.is_digimon])

            # Select and use 1 [TS] Option from hand with use cost <= opponent's Digimon count
            def option_filter(c):
                if not getattr(c, 'is_option', False):
                    return False
                if not any('TS' in _t for _t in (getattr(c, 'card_traits', []) or [])):
                    return False
                cost = c.get_cost_itself if hasattr(c, 'get_cost_itself') else getattr(c, 'play_cost', 0)
                return cost <= opp_digi_count

            game.effect_play_from_zone(
                player, 'hand', option_filter, free=True, is_optional=True,
                prompt="Select a [TS] Option to use for free.")

            # Then 1 [TS] Digimon may attack.
            # Engine limitation: no clean "optional end-of-turn attack" API.
            # Use FORCE_ATTACK modifier — if memory swings back to positive after
            # OnEndTurn effects, the game returns to Main Phase where the forced
            # attack will execute. If memory stays negative, turn ends (attack lost).
            # FORCE_ATTACK is mandatory not optional — engine gap for "may attack".
            from ....interfaces.modifiers import ModifierType
            def _ts_digi_filter(p):
                if not p.is_digimon:
                    return False
                top = getattr(p, 'top_card', None)
                if not top:
                    return False
                traits = getattr(top, 'card_traits', []) or []
                return any('TS' in t for t in traits)

            def on_attacker_selected(target_perm):
                if target_perm.is_suspended:
                    target_perm.unsuspend()
                game.register_modifier(
                    target_perm, ModifierType.FORCE_ATTACK,
                    value_fn=lambda: True, expiry='end_of_turn')

            game.effect_select_own_permanent(
                player, on_attacker_selected,
                filter_fn=_ts_digi_filter,
                is_optional=True,
                prompt="Select 1 of your [TS] Digimon that may attack.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Security] Play this card without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("BT24-085 Security: Play this card")
        effect3.set_effect_description("[Security] Play this card without paying the cost.")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
